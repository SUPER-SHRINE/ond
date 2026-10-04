use super::*;
use crate::{LoadedProject, diagnostic::Diagnostic, ir::hir};
use std::collections::BTreeSet;

pub fn validate_registry(cases: &[Case]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for case in cases {
        if case.id.is_empty() || !ids.insert(case.id) {
            return Err(format!("empty/duplicate case ID: {}", case.id));
        }
        validate(case).map_err(|e| format!("{}: {e}", case.id))?;
    }
    Ok(())
}

fn validate(case: &Case) -> Result<(), String> {
    if case.id.is_empty() || case.specs.is_empty() || case.specs.iter().any(|s| s.is_empty()) {
        return Err("case ID and spec IDs are required".into());
    }
    let mut paths = BTreeSet::new();
    for spec in case.specs {
        if !include_str!("../../docs/conformance-checklist.md").contains(&format!("| {spec} |")) {
            return Err(format!("unknown checklist spec ID: {spec}"));
        }
    }
    for file in case.files {
        // Portable relative paths, no traversal, Windows drives/ADS or aliases.
        if !file.path.ends_with(".ond")
            || file
                .path
                .contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
            || file.path.split('/').any(|part| {
                part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' '])
            })
            || file.path.split('/').any(|part| {
                let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
                matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                    || (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && stem.len() == 4
                        && stem.as_bytes()[3].is_ascii_digit()
            })
            || !paths.insert(file.path.to_ascii_lowercase())
        {
            return Err(format!("unsafe/duplicate source path: {}", file.path));
        }
    }
    match case.expected {
        Expected::Accept(checks) => {
            let executes = checks
                .iter()
                .any(|c| matches!(c, Check::MemoryExecution { .. }));
            if executes != (case.layer == Layer::MemoryVm) {
                return Err(
                    "MemoryVm layer requires execution checks; execution checks require MemoryVm"
                        .into(),
                );
            }
            if case.layer == Layer::Parse && checks.iter().any(|c| !matches!(c, Check::Ast { .. }))
            {
                return Err("parse cases cannot assert HIR/MIR".into());
            }
        }
        Expected::Reject(diagnostics) => {
            if diagnostics.is_empty() {
                return Err("rejection requires expected diagnostics".into());
            }
            for diagnostic in diagnostics {
                if let Location::EndOfFile { file } = diagnostic.location {
                    if !case.files.iter().any(|source| source.path == file) {
                        return Err("diagnostic source file not in fixture".into());
                    }
                }
                let message = match diagnostic.message {
                    Message::Exact(s) | Message::Contains(s) => s,
                };
                if message.is_empty() {
                    return Err("diagnostic message must not be empty".into());
                }
                if let Location::Source {
                    file,
                    text,
                    occurrence,
                } = diagnostic.location
                {
                    let source = case
                        .files
                        .iter()
                        .find(|s| s.path == file)
                        .ok_or("diagnostic source file not in fixture")?;
                    if text.is_empty() || source.text.match_indices(text).nth(occurrence).is_none()
                    {
                        return Err("diagnostic text occurrence not found".into());
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn run(case: &Case) -> Result<(), String> {
    run_inner(case).map_err(|error| {
        format!(
            "case={} specs={:?} layer={:?}: {error}",
            case.id, case.specs, case.layer
        )
    })
}

fn run_inner(case: &Case) -> Result<(), String> {
    validate(case)?;
    let fixture = fixture::Fixture::new(case.files, case.manifest)?;
    let loaded = match fixture.load() {
        Ok(loaded) => loaded,
        Err(error) => return reject(case, None, error.diagnostics()),
    }; // Keep SourceDb on failure; never assume FileId order.
    match case.layer {
        Layer::Parse => match crate::ond::parse_project(&loaded) {
            Ok(ast) => match case.expected {
                Expected::Reject(_) => Err("expected rejection, parsing succeeded".into()),
                Expected::Accept(checks) => {
                    for check in checks {
                        if let Check::Ast {
                            description,
                            verify,
                        } = check
                        {
                            verify(&ast).map_err(|e| format!("AST {description}: {e}"))?;
                        }
                    }
                    Ok(())
                }
            },
            Err(error) => reject(case, Some(&loaded), error.diagnostics()),
        },
        Layer::Core | Layer::MemoryVm => match crate::compile_loaded_project(loaded.clone()) {
            Ok(compilation) => match case.expected {
                Expected::Reject(_) => Err("expected rejection, compilation succeeded".into()),
                Expected::Accept(checks) => {
                    crate::mir::validate_project(&compilation.mir)
                        .map_err(|e| format!("MIR validator: {e:?}"))?;
                    for check in checks {
                        check_success(check, &compilation)?;
                    }
                    Ok(())
                }
            },
            Err(error) => reject(case, Some(&loaded), error.diagnostics()),
        },
    }
}

fn reject(
    case: &Case,
    loaded: Option<&LoadedProject>,
    actual: &[Diagnostic],
) -> Result<(), String> {
    if actual.iter().any(|d| d.message.contains("internal MIR")) {
        return Err(format!(
            "internal compiler error is not a source rejection: {actual:?}"
        ));
    }
    let Expected::Reject(expected) = case.expected else {
        return Err(format!("expected acceptance, got {actual:?}"));
    };
    if actual.len() != expected.len() {
        return Err(format!(
            "diagnostic count: expected {}, got {}: {actual:?}",
            expected.len(),
            actual.len()
        ));
    }
    // One-to-one matching, including overlapping Exact/Contains expectations.
    // A greedy first match would make acceptance depend on diagnostic order.
    let matches: Vec<Vec<bool>> = expected
        .iter()
        .map(|want| {
            actual
                .iter()
                .map(|got| diagnostic_matches(want, got, case, loaded))
                .collect()
        })
        .collect();
    let mut assigned = vec![None; actual.len()];
    for i in 0..expected.len() {
        if !assign(i, &matches, &mut assigned, &mut vec![false; actual.len()]) {
            return Err(format!(
                "expected diagnostic message/location not found: {actual:?}"
            ));
        }
    }
    Ok(())
}

fn assign(
    want: usize,
    matches: &[Vec<bool>],
    assigned: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for got in 0..assigned.len() {
        if !seen[got] && matches[want][got] {
            seen[got] = true;
            if assigned[got].is_none_or(|previous| assign(previous, matches, assigned, seen)) {
                assigned[got] = Some(want);
                return true;
            }
        }
    }
    false
}

fn diagnostic_matches(
    want: &ExpectedDiagnostic,
    got: &Diagnostic,
    case: &Case,
    loaded: Option<&LoadedProject>,
) -> bool {
    let message = match want.message {
        Message::Exact(s) => got.message == s,
        Message::Contains(s) => got.message.contains(s),
    };
    if got.severity != want.severity || !message {
        return false;
    }
    match want.location {
        Location::Project => got.primary.span.is_synthetic(),
        Location::EndOfFile { file } => {
            let Some(loaded) = loaded else {
                return false;
            };
            let Some(source) = loaded.sources.files().get(got.primary.span.file.0) else {
                return false;
            };
            let original = case.files.iter().find(|s| s.path == file).unwrap();
            source.path() == loaded.root.join(file)
                && got.primary.span.start == original.text.len()
                && got.primary.span.end == original.text.len()
        }
        Location::Source {
            file,
            text,
            occurrence,
        } => {
            let Some(loaded) = loaded else {
                return false;
            };
            let Some(source) = loaded.sources.files().get(got.primary.span.file.0) else {
                return false;
            };
            let Ok(relative) = source.path().strip_prefix(&loaded.root) else {
                return false;
            };
            let original = case.files.iter().find(|s| s.path == file).unwrap();
            let start = original.text.match_indices(text).nth(occurrence).unwrap().0;
            relative.to_string_lossy().replace('\\', "/") == file
                && got.primary.span.start == start
                && got.primary.span.end == start + text.len()
        }
    }
}

fn check_success(check: &Check, c: &crate::Compilation) -> Result<(), String> {
    match check {
        Check::MemoryExecution {
            description,
            function,
            input,
            expected,
        } => {
            let actual = crate::memory_test_vm::execute(&c.mir, function, input)
                .map_err(|e| format!("MemoryVm {description}: {e}"))?;
            if actual == *expected {
                Ok(())
            } else {
                Err(format!(
                    "MemoryVm {description}: expected {expected:?}, got {actual:?}"
                ))
            }
        }
        Check::Ast {
            description,
            verify,
        } => verify(&c.ast).map_err(|e| format!("AST {description}: {e}")),
        Check::Mir {
            description,
            verify,
        } => verify(&c.mir).map_err(|e| format!("MIR {description}: {e}")),
        Check::HirConstant {
            package,
            name,
            ty,
            value,
        } => {
            let p = c
                .hir
                .packages
                .iter()
                .find(|p| p.logical_path == *package)
                .ok_or_else(|| format!("missing package {package}"))?;
            let expr = p
                .items
                .iter()
                .find_map(|item| match item {
                    hir::Item::Const(item) => item
                        .names
                        .iter()
                        .position(|n| n == name)
                        .map(|i| &item.values[i]),
                    _ => None,
                })
                .ok_or_else(|| format!("missing constant {package}:{name}"))?;
            let actual_type = c.hir.types.display(expr.ty).to_string();
            if actual_type != *ty {
                return Err(format!("{name} type: expected {ty}, got {actual_type}"));
            }
            let matches = match (value, &expr.kind) {
                (Scalar::Integer(a), hir::ExprKind::Integer(b)) => a == b,
                (Scalar::Float32(a), hir::ExprKind::Float32(b)) => a == b,
                (Scalar::Bool(a), hir::ExprKind::Bool(b)) => a == b,
                (Scalar::Bytes(expected), hir::ExprKind::Composite(values)) => {
                    values.len() == expected.len()
                        && values.iter().enumerate().all(|(index, (key, value))| {
                            *key == index as u32
                                && matches!(value.kind, hir::ExprKind::Integer(byte) if byte == u64::from(expected[index]))
                        })
                }
                (Scalar::Nil, hir::ExprKind::Nil) => true,
                _ => false,
            };
            if matches {
                Ok(())
            } else {
                Err(format!("{name} constant value mismatch: {:?}", expr.kind))
            }
        }
    }
}
