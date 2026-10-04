//! Type/value boundary matrices. Expectations are specification data, not compiler queries.
mod builtins;
mod conversions;
mod declarations;
mod operators;
use super::*;

const TYPES: &[&str] = &[
    "u8", "i8", "u16", "i16", "u32", "i32", "f32", "bool", "*u8", "*i32", "func()", "[2]i32", "S",
    "Count",
];
const SETUP: &str = "type S struct { x: i32 }\ntype Count i32\n";
const IDENTICAL: &str = "type mismatch";
fn integer(ty: &str) -> bool {
    matches!(ty, "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "Count")
}

struct Row {
    id: String,
    specs: &'static [&'static str],
    source: String,
    error: Option<(&'static str, String)>,
    occurrence: Option<usize>,
    checks: Vec<Check>,
}
impl Row {
    fn new(id: impl Into<String>, specs: &'static [&'static str], code: impl AsRef<str>) -> Self {
        Self {
            id: id.into(),
            specs,
            source: format!("package main\n{SETUP}{}\nfunc main() {{}}\n", code.as_ref()),
            error: None,
            occurrence: None,
            checks: Vec::new(),
        }
    }
    fn reject(mut self, message: &'static str, span: impl Into<String>) -> Self {
        self.error = Some((message, span.into()));
        self
    }
    fn run(&self) {
        let diagnostics;
        let expected = if let Some((message, span)) = &self.error {
            diagnostics = [ExpectedDiagnostic {
                severity: Severity::Error,
                message: Message::Contains(message),
                location: Location::Source {
                    file: "main.ond",
                    text: span,
                    occurrence: self.occurrence.unwrap_or_else(|| {
                        self.source.match_indices(span).count().saturating_sub(1)
                    }),
                },
            }];
            Expected::Reject(&diagnostics)
        } else {
            Expected::Accept(&self.checks)
        };
        run(&Case {
            id: &self.id,
            specs: self.specs,
            layer: Layer::Core,
            manifest: true,
            files: &[SourceFile {
                path: "main.ond",
                text: &self.source,
            }],
            expected,
        })
        .unwrap_or_else(|e| panic!("{e}\nsource:\n{}", self.source));
    }
    fn check(mut self, check: Check) -> Self {
        self.checks.push(check);
        self
    }
}
fn run_rows(rows: Vec<Row>) {
    for row in rows {
        row.run();
    }
}
pub(super) fn ids() -> Vec<String> {
    [
        conversions::rows(),
        operators::rows(),
        builtins::rows(),
        declarations::rows(),
    ]
    .into_iter()
    .flatten()
    .map(|r| r.id)
    .collect()
}
#[test]
fn conversions_and_nil() {
    run_rows(conversions::rows());
}
#[test]
fn operator_operand_types() {
    run_rows(operators::rows());
}
#[test]
fn builtin_boundaries() {
    run_rows(builtins::rows());
}
#[test]
fn declaration_boundaries() {
    run_rows(declarations::rows());
}
#[test]
fn matrix_inventory() {
    assert_eq!(TYPES.len(), 14);
    for (name, rows, expected) in [
        ("conversions and nil", conversions::rows(), 866),
        ("operators", operators::rows(), 322),
        ("builtins", builtins::rows(), 117),
        ("declarations", declarations::rows(), 61),
    ] {
        assert_eq!(
            rows.len(),
            expected,
            "{name}: update the inventory with the matrix"
        );
        assert!(
            rows.iter().any(|r| r.error.is_some()),
            "{name}: missing negative controls"
        );
        assert!(
            rows.iter().any(|r| r.error.is_none()),
            "{name}: missing positive controls"
        );
    }
    assert_eq!(ids().len(), 1366);
}
