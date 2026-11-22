//! Reject oversized arrays and structs after recursive types and generic instances resolve.
use crate::{
    diagnostic::{Diagnostic, Diagnostics},
    project::PackageId,
    semantic::{TypeId, TypeKind, TypeTable},
    source::Span,
};
use std::collections::{BTreeMap, HashSet};

use super::{generic_functions, types::TypePackage};

// Sizes above the limit are indistinguishable for aggregate acceptance. Capping at
// 2^32 preserves alignment (at most four) and lets zero-length arrays stay zero.
const TOO_LARGE: u64 = u32::MAX as u64 + 1;

pub(super) fn record_struct_site(
    types: &mut TypeTable,
    universe: &[TypePackage],
    package_id: PackageId,
    id: TypeId,
    span: Span,
) {
    if !types.records_struct_layout_sites() {
        return;
    }
    let mut request = universe
        .iter()
        .find(|package| package.id == package_id)
        .and_then(|package| package.generic_type_request(span));
    let mut requests = Vec::new();
    // An inner request can be written in another active generic declaration.
    while let Some(span) = request {
        if requests.contains(&span) {
            break;
        }
        requests.push(span);
        request = universe
            .iter()
            .find_map(|package| package.generic_type_request(span));
    }
    // Interning ignores spans. Keep each use's diagnostic origin separately.
    types.record_struct_layout_site(id, span, requests);
}

pub(super) fn validate(
    types: &mut TypeTable,
    universe: &[TypePackage],
    diagnostics: &mut Diagnostics,
) {
    let mut layouts = BTreeMap::new();
    for definition in types.definitions() {
        if matches!(definition.kind, TypeKind::Array { .. })
            && bounded_layout(types, definition.id, &mut layouts)
                .is_some_and(|(size, _)| size == TOO_LARGE)
        {
            diagnostics.push(
                Diagnostic::error(definition.span, "array layout overflow")
                    .with_note("fixed-size array size must not exceed 4294967295 bytes"),
            );
        }
    }
    let mut reported = HashSet::new();
    let mut rejected_structs = HashSet::new();
    let mut struct_errors = Diagnostics::new();
    let mut specialization_errors = Vec::new();
    for site in types.take_struct_layout_sites() {
        if bounded_layout(types, site.id, &mut layouts).is_some_and(|(size, _)| size == TOO_LARGE)
            && reported.insert(site.clone())
        {
            rejected_structs.insert(site.id);
            let mut diagnostic = Diagnostic::error(site.span, "struct layout overflow")
                .with_note("struct size including padding must not exceed 4294967295 bytes");
            for request in site.requests {
                diagnostic = diagnostic.reanchor(request, "while instantiating this generic type");
            }
            if let Some(symbol) = site.specialization {
                specialization_errors.push((symbol, diagnostic));
            } else {
                struct_errors.push(diagnostic);
            }
        }
    }
    // A failed query stops expression lowering immediately. Prefer the type
    // diagnostics when the same layout was rejected here at its source sites.
    let covered_queries = types
        .take_struct_layout_queries()
        .into_iter()
        .filter_map(|(id, span)| rejected_structs.contains(&id).then_some(span))
        .collect::<HashSet<_>>();
    if !covered_queries.is_empty() {
        diagnostics.retain(|diagnostic| {
            diagnostic.message != "struct layout overflow"
                || !std::iter::once(&diagnostic.primary)
                    .chain(&diagnostic.related)
                    .any(|label| covered_queries.contains(&label.span))
        });
    }
    diagnostics.extend(struct_errors);
    generic_functions::report_layout_diagnostics(universe, diagnostics, specialization_errors);
}

fn bounded_layout(
    types: &TypeTable,
    id: TypeId,
    layouts: &mut BTreeMap<TypeId, Option<(u64, u32)>>,
) -> Option<(u64, u32)> {
    if let Some(layout) = layouts.get(&id) {
        return *layout;
    }
    // Mark active types as unknown so invalid value cycles keep their existing
    // diagnostics. Completed types are shared across every aggregate reference.
    layouts.insert(id, None);
    let layout = match types.underlying_kind(id)? {
        TypeKind::Bool | TypeKind::U8 | TypeKind::I8 => Some((1, 1)),
        TypeKind::U16 | TypeKind::I16 => Some((2, 2)),
        TypeKind::U32
        | TypeKind::I32
        | TypeKind::F32
        | TypeKind::Q16
        | TypeKind::Pointer(_)
        | TypeKind::Function(_) => Some((4, 4)),
        TypeKind::U64 | TypeKind::I64 | TypeKind::F64 | TypeKind::Q32 => Some((8, 4)),
        TypeKind::Array { length, element } => {
            let (size, alignment) = bounded_layout(types, *element, layouts)?;
            Some(((size * u64::from(*length)).min(TOO_LARGE), alignment))
        }
        TypeKind::Struct(structure) => {
            let mut size = 0u64;
            let mut alignment = 1u32;
            for field in &structure.fields {
                let (field_size, field_alignment) = bounded_layout(types, field.ty, layouts)?;
                size = align(size, field_alignment);
                size = (size + field_size).min(TOO_LARGE);
                alignment = alignment.max(field_alignment);
            }
            Some((align(size, alignment), alignment))
        }
        TypeKind::Pending | TypeKind::Defined { .. } => None,
    };
    layouts.insert(id, layout);
    layout
}

fn align(size: u64, alignment: u32) -> u64 {
    let mask = u64::from(alignment - 1);
    ((size + mask) & !mask).min(TOO_LARGE)
}
