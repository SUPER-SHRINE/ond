//! Kagura relocation and contiguous-memory layout, independent of MIR and container encoding.
use crate::ir::object::{Object, RelocationKind, SectionKind};
use ond_protocol::debug::{DebugInfo, DebugObject, DebugRange, FunctionDebug, UnwindRow};
use std::collections::BTreeMap;
mod symbols;
#[cfg(test)]
mod tests;
mod thunks;
mod validate;

pub use ond_protocol::link::{LinkPlan, LinkedImage, LinkedSection, MemoryRegion};

#[derive(Debug, Clone)]
pub struct LinkedOutput {
    pub image: LinkedImage,
    pub debug: DebugInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkError {
    pub message: String,
}
impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for LinkError {}
fn link_error(message: impl Into<String>) -> LinkError {
    LinkError {
        message: message.into(),
    }
}

pub fn link_objects(objects: &[Object], plan: &LinkPlan) -> Result<LinkedImage, LinkError> {
    Ok(link_objects_with_debug(objects, &[], plan)?.image)
}

pub fn link_objects_with_debug(
    objects: &[Object],
    debug_objects: &[DebugObject],
    plan: &LinkPlan,
) -> Result<LinkedOutput, LinkError> {
    validate::validate(objects)?;
    let ram_size = plan.memory_size;
    let stack_size = plan.stack_size;
    let memory_end = plan
        .memory_base
        .checked_add(ram_size)
        .ok_or_else(|| link_error("memory region address overflow"))?;
    if ram_size == 0 || stack_size == 0 || stack_size >= ram_size {
        return Err(link_error("invalid RAM/stack configuration"));
    }
    if plan.memory_base % 4 != 0 || memory_end % 4 != 0 || stack_size % 4 != 0 {
        return Err(link_error("memory/stack must be word-aligned"));
    }
    let readonly_end = if let Some(r) = plan.read_only {
        let end = r
            .base
            .checked_add(r.size)
            .ok_or_else(|| link_error("read-only address overflow"))?;
        if r.size == 0
            || r.base % 4 != 0
            || end % 4 != 0
            || (r.base < memory_end && end > plan.memory_base)
        {
            return Err(link_error("invalid or overlapping read-only region"));
        }
        Some(end)
    } else {
        None
    };
    let mut section_offsets = BTreeMap::<(usize, SectionKind), u32>::new();
    let mut linked_sections = Vec::<LinkedSection>::new();

    for kind in [
        SectionKind::Text,
        SectionKind::Rodata,
        SectionKind::Data,
        SectionKind::Bss,
    ] {
        let inputs = objects
            .iter()
            .enumerate()
            .filter_map(|(index, object)| {
                object
                    .sections
                    .iter()
                    .find(|section| section.kind == kind)
                    .map(|section| (index, section))
            })
            .collect::<Vec<_>>();
        if inputs.is_empty() {
            continue;
        }

        let alignment = inputs
            .iter()
            .map(|(_, section)| section.alignment.max(4))
            .max()
            .unwrap_or(4);
        let mut cursor = 0u32;
        let mut data = Vec::new();

        for (index, section) in inputs {
            cursor = align_up(cursor, section.alignment.max(4))?;
            let next = cursor
                .checked_add(section.memory_size)
                .ok_or_else(|| link_error("linked section size overflow"))?;
            let capacity = if matches!(kind, SectionKind::Text | SectionKind::Rodata) {
                plan.read_only.map_or(ram_size - stack_size, |r| r.size)
            } else {
                ram_size - stack_size
            };
            if kind != SectionKind::Bss && next > capacity {
                return Err(link_error("section does not fit in target region"));
            }
            if kind != SectionKind::Bss {
                if cursor as usize > data.len() {
                    data.resize(cursor as usize, 0);
                }
                data.extend_from_slice(&section.data);
                if kind == SectionKind::Data && section.memory_size as usize > section.data.len() {
                    data.resize(
                        data.len() + (section.memory_size as usize - section.data.len()),
                        0,
                    );
                }
            }
            section_offsets.insert((index, kind), cursor);
            cursor = cursor
                .checked_add(section.memory_size)
                .ok_or_else(|| link_error("linked section size overflow"))?;
        }

        linked_sections.push(LinkedSection {
            kind,
            alignment,
            load_address: 0,
            memory_size: cursor,
            data,
        });
    }

    let mut load_cursor = plan.memory_base;
    let mut rom_cursor = plan.read_only.map_or(0, |r| r.base);
    for section in &mut linked_sections {
        if plan.read_only.is_some()
            && matches!(section.kind, SectionKind::Text | SectionKind::Rodata)
        {
            rom_cursor = align_up(rom_cursor, section.alignment)?;
            section.load_address = rom_cursor;
            rom_cursor = rom_cursor
                .checked_add(section.memory_size)
                .ok_or_else(|| link_error("read-only size overflow"))?;
            if rom_cursor > readonly_end.unwrap() {
                return Err(link_error("read-only image does not fit"));
            }
            continue;
        }
        load_cursor = align_up(load_cursor, section.alignment)?;
        section.load_address = load_cursor;
        load_cursor = load_cursor
            .checked_add(section.memory_size)
            .ok_or_else(|| link_error("linked image address overflow"))?;
    }

    let stack_bottom = plan
        .memory_base
        .checked_add(ram_size)
        .and_then(|end| end.checked_sub(stack_size))
        .ok_or_else(|| link_error("invalid RAM/stack configuration"))?;
    if load_cursor > stack_bottom {
        return Err(link_error("final image does not fit before stack"));
    }

    let resolved = symbols::Symbols::build(
        objects,
        &section_offsets,
        &linked_sections,
        plan,
        load_cursor,
        stack_bottom,
    )?;
    let mut slot_interiors = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        for r in &o.relocations {
            if matches!(r.kind, RelocationKind::CallSlot | RelocationKind::JumpSlot) {
                let base = linked_sections
                    .iter()
                    .find(|s| s.kind == r.section)
                    .unwrap()
                    .load_address;
                let start = base + section_offsets[&(i, r.section)] + r.offset;
                slot_interiors.push((start, start + 24));
            }
        }
    }

    let text_ranges: Vec<_> = linked_sections
        .iter()
        .filter(|s| s.kind == SectionKind::Text)
        .map(|s| (s.load_address, s.load_address + s.memory_size))
        .collect();
    for (object_index, object) in objects.iter().enumerate() {
        for relocation in &object.relocations {
            let section_offset = section_offsets
                .get(&(object_index, relocation.section))
                .copied()
                .unwrap_or(0);
            let linked_section = linked_sections
                .iter_mut()
                .find(|section| section.kind == relocation.section)
                .ok_or_else(|| link_error("relocation refers to missing section"))?;
            let patch_offset = section_offset
                .checked_add(relocation.offset)
                .ok_or_else(|| link_error("relocation offset overflow"))?;
            let target = resolved.resolve(object_index, &relocation.target)?;
            let effective = checked_address(target, relocation.addend)?;
            if slot_interiors
                .iter()
                .any(|&(a, b)| effective > a && effective < b)
            {
                return Err(link_error("relocation target enters branch slot interior"));
            }
            if relocation.kind != RelocationKind::Abs32
                && !text_ranges
                    .iter()
                    .any(|&(a, b)| effective >= a && effective < b)
            {
                return Err(link_error("branch target is outside executable text"));
            }
            match relocation.kind {
                RelocationKind::CallSlot | RelocationKind::JumpSlot => {
                    let target = resolved.resolve(object_index, &relocation.target)?;
                    thunks::patch(object, relocation, linked_section, patch_offset, target)?;
                }
                RelocationKind::Call16 | RelocationKind::Jump16 => {
                    let target = resolved.resolve(object_index, &relocation.target)?;
                    patch_branch_imm(linked_section, patch_offset, target, relocation.addend)?;
                }
                RelocationKind::Abs32 => {
                    let target = resolved.resolve(object_index, &relocation.target)?;
                    patch_u32(
                        linked_section,
                        patch_offset,
                        checked_address(target, relocation.addend)?,
                    )?;
                }
            }
        }
    }

    if !objects.iter().flat_map(|o| &o.symbols).any(|s| {
        s.name == plan.entry_symbol
            && s.binding == crate::ir::object::SymbolBinding::Exported
            && s.kind == crate::ir::object::SymbolKind::Function
    }) {
        return Err(link_error("missing executable exported entry symbol"));
    }
    let entry_point = resolved
        .exports
        .get(&plan.entry_symbol)
        .copied()
        .ok_or_else(|| link_error("missing startup entry symbol"))?;

    let mut functions = Vec::new();
    let mut ranges = Vec::new();
    let mut debug_packages = std::collections::BTreeSet::new();
    for debug in debug_objects {
        if !debug_packages.insert(&debug.package) {
            return Err(link_error(format!(
                "duplicate debug object: {}",
                debug.package
            )));
        }
        let object_index = objects
            .iter()
            .position(|object| object.name == debug.package)
            .ok_or_else(|| link_error(format!("debug data has no object: {}", debug.package)))?;
        for function in &debug.functions {
            validate_debug_extent(
                &objects[object_index],
                function.section,
                function.offset,
                function
                    .offset
                    .checked_add(function.size)
                    .ok_or_else(|| link_error("debug function extent overflow"))?,
            )?;
            let base = debug_base(
                object_index,
                function.section,
                &section_offsets,
                &linked_sections,
            )?;
            let address = base
                .checked_add(function.offset)
                .ok_or_else(|| link_error("debug function address overflow"))?;
            functions.push(FunctionDebug {
                symbol: function.symbol.clone(),
                address,
                size: function.size,
                frame_size: function.frame_size,
                return_address_offset: function.return_address_offset,
                source: function.source.clone(),
                unwind: function
                    .unwind
                    .iter()
                    .map(|row| {
                        validate_debug_extent(
                            &objects[object_index],
                            function.section,
                            row.offset,
                            row.offset,
                        )?;
                        Ok(UnwindRow {
                            address: base
                                .checked_add(row.offset)
                                .ok_or_else(|| link_error("debug unwind address overflow"))?,
                            cfa_sp_offset: row.cfa_sp_offset,
                            return_address: row.return_address,
                        })
                    })
                    .collect::<Result<Vec<_>, LinkError>>()?,
            });
        }
        for range in &debug.ranges {
            validate_debug_extent(
                &objects[object_index],
                range.section,
                range.start,
                range.end,
            )?;
            let base = debug_base(
                object_index,
                range.section,
                &section_offsets,
                &linked_sections,
            )?;
            ranges.push(DebugRange {
                start: base
                    .checked_add(range.start)
                    .ok_or_else(|| link_error("debug range address overflow"))?,
                end: base
                    .checked_add(range.end)
                    .ok_or_else(|| link_error("debug range address overflow"))?,
                operation: range.operation,
                message: range.message.clone(),
                source: range.source.clone(),
                function: range.function.clone(),
            });
        }
    }
    functions.sort_by_key(|function| function.address);
    ranges.sort_by_key(|range| (range.start, range.end));

    let image = LinkedImage {
        build_id: String::new(),
        entry_point,
        ram_size,
        stack_size,
        sections: linked_sections,
        heap_base: load_cursor,
        stack_bottom,
        stack_top: memory_end,
        symbols: resolved.exports,
    };
    let debug = DebugInfo {
        build_id: String::new(),
        stack_bottom: image.stack_bottom,
        stack_top: image.stack_top,
        functions,
        ranges,
    };
    Ok(LinkedOutput { image, debug })
}

fn validate_debug_extent(
    object: &Object,
    section: SectionKind,
    start: u32,
    end: u32,
) -> Result<(), LinkError> {
    let size = object
        .sections
        .iter()
        .find(|candidate| candidate.kind == section)
        .map(|candidate| candidate.memory_size)
        .ok_or_else(|| link_error("debug data refers to a missing object section"))?;
    if start > end || end > size {
        return Err(link_error("debug data is outside its object section"));
    }
    Ok(())
}

fn debug_base(
    object_index: usize,
    section: SectionKind,
    section_offsets: &BTreeMap<(usize, SectionKind), u32>,
    linked_sections: &[LinkedSection],
) -> Result<u32, LinkError> {
    let linked = linked_sections
        .iter()
        .find(|candidate| candidate.kind == section)
        .ok_or_else(|| link_error("debug data refers to a missing linked section"))?;
    linked
        .load_address
        .checked_add(
            section_offsets
                .get(&(object_index, section))
                .copied()
                .ok_or_else(|| link_error("debug data refers to a missing object section"))?,
        )
        .ok_or_else(|| link_error("debug section address overflow"))
}

fn patch_branch_imm(
    section: &mut LinkedSection,
    patch_offset: u32,
    target_address: u32,
    addend: i32,
) -> Result<(), LinkError> {
    let site_address = section
        .load_address
        .checked_add(patch_offset)
        .ok_or_else(|| link_error("branch patch address overflow"))?;
    let target_address = checked_address(target_address, addend)?;
    let diff = i64::from(target_address) - i64::from(site_address.wrapping_add(4));
    if diff % 4 != 0 {
        return Err(link_error("branch relocation target is not word-aligned"));
    }
    let imm = diff / 4;
    if !(i16::MIN as i64..=i16::MAX as i64).contains(&imm) {
        return Err(link_error("branch relocation out of range"));
    }
    let word = read_u32(section, patch_offset)?;
    let patched = (word & 0xFFFF_0000) | ((imm as i16 as u16) as u32);
    write_u32(section, patch_offset, patched)
}

fn patch_u32(section: &mut LinkedSection, patch_offset: u32, value: u32) -> Result<(), LinkError> {
    write_u32(section, patch_offset, value)
}

pub(crate) fn read_u32(section: &LinkedSection, offset: u32) -> Result<u32, LinkError> {
    let start = offset as usize;
    let bytes = section
        .data
        .get(start..start + 4)
        .ok_or_else(|| link_error("relocation patch out of bounds"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn write_u32(section: &mut LinkedSection, offset: u32, value: u32) -> Result<(), LinkError> {
    let start = offset as usize;
    let bytes = section
        .data
        .get_mut(start..start + 4)
        .ok_or_else(|| link_error("relocation patch out of bounds"))?;
    bytes.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn align_up(value: u32, alignment: u32) -> Result<u32, LinkError> {
    if alignment <= 1 {
        Ok(value)
    } else {
        let rounded = u64::from(value).div_ceil(u64::from(alignment)) * u64::from(alignment);
        u32::try_from(rounded).map_err(|_| link_error("linked alignment overflow"))
    }
}
fn checked_address(target: u32, addend: i32) -> Result<u32, LinkError> {
    u32::try_from(i64::from(target) + i64::from(addend))
        .map_err(|_| link_error("relocation target address overflow"))
}
