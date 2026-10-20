//! Writable globals always occupy BSS; constant initialization executes at startup.
use super::*;
pub(super) fn storage(
    types: &mir::TypeTable,
    package: &mir::Package,
) -> Result<(Section, Section, Vec<Symbol>, Vec<Relocation>), Diagnostic> {
    let mut cursor = 0u32;
    let mut symbols = Vec::new();
    for global in &package.globals {
        let data = layout::data(types, global.ty, global.span)?;
        cursor = layout::align(cursor, data.alignment, global.span)?;
        let offset = cursor;
        cursor = cursor
            .checked_add(data.size.max(1))
            .ok_or_else(|| error(global.span, "global storage overflow"))?;
        symbols.push(Symbol {
            name: global.symbol.clone(),
            binding: SymbolBinding::Exported,
            kind: SymbolKind::Object,
            section: Some(SectionKind::Bss),
            offset,
            size: data.size,
        });
    }
    let span = package
        .globals
        .last()
        .map_or(Span::synthetic(), |global| global.span);
    Ok((
        Section {
            kind: SectionKind::Data,
            alignment: 4,
            memory_size: 0,
            data: Vec::new(),
        },
        Section {
            kind: SectionKind::Bss,
            alignment: 4,
            memory_size: layout::align(cursor, 4, span)?,
            data: Vec::new(),
        },
        symbols,
        Vec::new(),
    ))
}

pub(super) fn initializer(
    types: &mir::TypeTable,
    package: &mir::Package,
    emitter: &mut emit::Emitter,
) -> Result<(), Diagnostic> {
    for global in &package.globals {
        if let Some(value) = &global.initializer {
            emitter.symbol(2, &global.symbol, global.span)?;
            initialize_value(types, value, 0, global.span, emitter)?;
        }
    }
    emitter.emit(12, 0, 0, 15, 0);
    Ok(())
}

fn initialize_value(
    types: &mir::TypeTable,
    value: &mir::StaticValue,
    offset: u32,
    span: Span,
    emitter: &mut emit::Emitter,
) -> Result<(), Diagnostic> {
    use mir::StaticValueKind;
    let bits = match &value.kind {
        StaticValueKind::Zero | StaticValueKind::Nil => return Ok(()),
        StaticValueKind::Integer(bits) => *bits as u32,
        StaticValueKind::Float32(bits) => *bits,
        StaticValueKind::Bool(value) => u32::from(*value),
        StaticValueKind::Function { symbol, .. } => {
            emitter.symbol(1, symbol, span)?;
            memory::write(
                emitter,
                1,
                2,
                offset,
                layout::scalar(types, value.ty, span)?,
            );
            return Ok(());
        }
        StaticValueKind::Composite(elements) => {
            let data = layout::data(types, value.ty, span)?;
            for (index, child) in elements {
                let child_offset = match types.underlying_kind(value.ty) {
                    Some(mir::TypeKind::Array { element, .. }) => {
                        layout::data(types, *element, span)?
                            .size
                            .checked_mul(*index)
                    }
                    Some(mir::TypeKind::Struct(_)) => data.fields.get(*index as usize).copied(),
                    _ => None,
                }
                .and_then(|child_offset| offset.checked_add(child_offset))
                .ok_or_else(|| error(span, "global initializer offset overflow"))?;
                initialize_value(types, child, child_offset, span, emitter)?;
            }
            return Ok(());
        }
    };
    if bits != 0 {
        emitter.immediate(1, bits);
        memory::write(
            emitter,
            1,
            2,
            offset,
            layout::scalar(types, value.ty, span)?,
        );
    }
    Ok(())
}
