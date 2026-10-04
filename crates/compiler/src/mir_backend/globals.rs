//! Writable global storage. Constant initializers are encoded in Data; omitted
//! initializers remain zero-filled BSS and require no startup code.
use super::*;
pub(super) fn storage(
    types: &mir::TypeTable,
    package: &mir::Package,
) -> Result<(Section, Section, Vec<Symbol>, Vec<Relocation>), Diagnostic> {
    let mut data_cursor = 0u32;
    let mut bss_cursor = 0u32;
    let mut data = Vec::new();
    let mut symbols = Vec::new();
    let mut relocations = Vec::new();
    for global in &package.globals {
        let d = layout::data(types, global.ty, global.span)?;
        let (cursor, section) = if global.initializer.is_some() {
            (&mut data_cursor, SectionKind::Data)
        } else {
            (&mut bss_cursor, SectionKind::Bss)
        };
        *cursor = layout::align(*cursor, d.alignment, global.span)?;
        let offset = *cursor;
        // Physical identity is non-nil even when the logical type size is zero.
        *cursor = cursor
            .checked_add(d.size.max(1))
            .ok_or_else(|| error(global.span, "global storage overflow"))?;
        if let Some(initializer) = &global.initializer {
            data.resize(data_cursor as usize, 0);
            readonly::write_value(
                types,
                initializer,
                offset,
                &mut data,
                &mut relocations,
                SectionKind::Data,
                global.span,
            )?;
        }
        symbols.push(Symbol {
            name: global.symbol.clone(),
            binding: SymbolBinding::Exported,
            kind: SymbolKind::Object,
            section: Some(section),
            offset,
            size: d.size,
        });
    }
    let span = package.globals.last().map_or(Span::synthetic(), |g| g.span);
    let data_size = layout::align(data_cursor, 4, span)?;
    data.resize(data_size as usize, 0);
    let bss_size = layout::align(bss_cursor, 4, span)?;
    Ok((
        Section {
            kind: SectionKind::Data,
            alignment: 4,
            memory_size: data_size,
            data,
        },
        Section {
            kind: SectionKind::Bss,
            alignment: 4,
            memory_size: bss_size,
            data: Vec::new(),
        },
        symbols,
        relocations,
    ))
}
