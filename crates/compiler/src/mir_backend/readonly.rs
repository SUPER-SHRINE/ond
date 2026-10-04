//! Compile-time aggregate values stored once in the linked read-only image.
use super::*;
use std::collections::BTreeMap;

pub(super) fn storage(
    types: &mir::TypeTable,
    package: &mir::Package,
) -> Result<(Section, Vec<Symbol>, Vec<Relocation>), Diagnostic> {
    let mut cursor = 0u32;
    let mut alignment = 1u32;
    let mut data = Vec::new();
    let mut placements = BTreeMap::<&str, (u32, u32)>::new();
    let mut strings = Vec::<(u32, Vec<u8>)>::new();
    let mut relocations = Vec::new();

    let mut terminated = package
        .statics
        .iter()
        .filter(|item| item.null_terminated)
        .map(|item| {
            let item_layout = layout::data(types, item.value.ty, item.span)?;
            if item_layout.alignment != 1 {
                return Err(error(
                    item.span,
                    "null-terminated static must have byte alignment",
                ));
            }
            let mut bytes = vec![0; item_layout.size as usize];
            write_value(
                types,
                &item.value,
                0,
                &mut bytes,
                &mut relocations,
                SectionKind::Rodata,
                item.span,
            )?;
            bytes.push(0);
            Ok((item, item_layout.size, bytes))
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    terminated.sort_by(|(left, _, left_bytes), (right, _, right_bytes)| {
        right_bytes
            .len()
            .cmp(&left_bytes.len())
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    for (item, logical_size, bytes) in terminated {
        let offset = if let Some((base, stored)) =
            strings.iter().find(|(_, stored)| stored.ends_with(&bytes))
        {
            let suffix = stored.len() - bytes.len();
            let suffix = u32::try_from(suffix)
                .map_err(|_| error(item.span, "read-only data offset overflow"))?;
            base.checked_add(suffix)
                .ok_or_else(|| error(item.span, "read-only data offset overflow"))?
        } else {
            let offset = cursor;
            let stored_size = u32::try_from(bytes.len())
                .map_err(|_| error(item.span, "read-only data size overflow"))?;
            cursor = cursor
                .checked_add(stored_size)
                .ok_or_else(|| error(item.span, "read-only data size overflow"))?;
            data.extend_from_slice(&bytes);
            strings.push((offset, bytes));
            offset
        };
        placements.insert(&item.symbol, (offset, logical_size));
    }

    for item in package.statics.iter().filter(|item| !item.null_terminated) {
        let item_layout = layout::data(types, item.value.ty, item.span)?;
        cursor = layout::align(cursor, item_layout.alignment, item.span)?;
        alignment = alignment.max(item_layout.alignment);
        let offset = cursor;
        cursor = cursor
            .checked_add(item_layout.size.max(1))
            .ok_or_else(|| error(item.span, "read-only data size overflow"))?;
        data.resize(cursor as usize, 0);
        write_value(
            types,
            &item.value,
            offset,
            &mut data,
            &mut relocations,
            SectionKind::Rodata,
            item.span,
        )?;
        placements.insert(&item.symbol, (offset, item_layout.size));
    }

    let symbols = package
        .statics
        .iter()
        .map(|item| {
            let (offset, size) = placements[&item.symbol.as_str()];
            Symbol {
                name: item.symbol.clone(),
                binding: if item.exported {
                    SymbolBinding::Exported
                } else {
                    SymbolBinding::Local
                },
                kind: SymbolKind::Object,
                section: Some(SectionKind::Rodata),
                offset,
                size,
            }
        })
        .collect();

    Ok((
        Section {
            kind: SectionKind::Rodata,
            alignment,
            memory_size: cursor,
            data,
        },
        symbols,
        relocations,
    ))
}

pub(super) fn write_value(
    types: &mir::TypeTable,
    value: &mir::StaticValue,
    offset: u32,
    data: &mut [u8],
    relocations: &mut Vec<Relocation>,
    section: SectionKind,
    span: Span,
) -> Result<(), Diagnostic> {
    use mir::StaticValueKind as V;

    match &value.kind {
        V::Zero | V::Nil => Ok(()),
        V::Composite(values) => match types.underlying_kind(value.ty) {
            Some(mir::TypeKind::Array { length, element }) => {
                let stride = layout::data(types, *element, span)?.size;
                for (index, child) in values {
                    if *index >= *length || child.ty != *element {
                        return Err(error(span, "invalid static array initializer"));
                    }
                    write_value(
                        types,
                        child,
                        offset + index * stride,
                        data,
                        relocations,
                        section,
                        span,
                    )?;
                }
                Ok(())
            }
            Some(mir::TypeKind::Struct(structure)) => {
                let fields = layout::data(types, value.ty, span)?.fields;
                for (index, child) in values {
                    let Some(field) = structure.fields.get(*index as usize) else {
                        return Err(error(span, "invalid static struct initializer"));
                    };
                    if child.ty != field.ty {
                        return Err(error(span, "static struct field type mismatch"));
                    }
                    write_value(
                        types,
                        child,
                        offset + fields[*index as usize],
                        data,
                        relocations,
                        section,
                        span,
                    )?;
                }
                Ok(())
            }
            Some(mir::TypeKind::Interface(interface)) => {
                let fields = layout::data(types, value.ty, span)?.fields;
                let expected = [interface.data_pointer, interface.vtable];
                for (index, child) in values {
                    let Some(field_ty) = expected.get(*index as usize) else {
                        return Err(error(span, "invalid static interface initializer"));
                    };
                    if child.ty != *field_ty {
                        return Err(error(span, "static interface field type mismatch"));
                    }
                    write_value(
                        types,
                        child,
                        offset + fields[*index as usize],
                        data,
                        relocations,
                        section,
                        span,
                    )?;
                }
                Ok(())
            }
            _ => Err(error(
                span,
                "composite static initializer requires aggregate type",
            )),
        },
        V::Integer(bits) => write_scalar(types, value.ty, *bits as u32, offset, data, span),
        V::Float32(bits) => write_scalar(types, value.ty, *bits, offset, data, span),
        V::Bool(boolean) => write_scalar(types, value.ty, u32::from(*boolean), offset, data, span),
        V::Function { symbol, .. } => {
            relocations.push(Relocation {
                section,
                offset,
                kind: RelocationKind::Abs32,
                target: symbol.clone(),
                addend: 0,
            });
            Ok(())
        }
    }
}

fn write_scalar(
    types: &mir::TypeTable,
    ty: mir::TypeId,
    bits: u32,
    offset: u32,
    data: &mut [u8],
    span: Span,
) -> Result<(), Diagnostic> {
    let scalar = layout::scalar(types, ty, span)?;
    let bytes = bits.to_le_bytes();
    let start = offset as usize;
    let end = start + scalar.size as usize;
    let destination = data
        .get_mut(start..end)
        .ok_or_else(|| error(span, "static initializer exceeds read-only storage"))?;
    destination.copy_from_slice(&bytes[..scalar.size as usize]);
    Ok(())
}
