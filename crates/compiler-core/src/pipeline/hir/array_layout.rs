//! Reject oversized arrays after recursive types and generic instances resolve.
use crate::{
    diagnostic::{Diagnostic, Diagnostics},
    semantic::{TypeId, TypeKind, TypeTable},
};
use std::collections::BTreeMap;

// Sizes above the limit are indistinguishable for array acceptance. Capping at
// 2^32 preserves alignment (at most four) and lets zero-length arrays stay zero.
const TOO_LARGE: u64 = u32::MAX as u64 + 1;

pub(super) fn validate(types: &TypeTable, diagnostics: &mut Diagnostics) {
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
        | TypeKind::Pointer(_)
        | TypeKind::Function(_) => Some((4, 4)),
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
