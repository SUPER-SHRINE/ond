use super::*;
use ond_protocol::executable::{Executable, ExecutableSection};

/// Fixed header location is supplied by the linker; no host register setup is needed.
pub fn self_boot_prefix(kinds: &[SectionKind], header_address: u32) -> Vec<u8> {
    use crate::kagura_encoding::{literal_load, word};
    let mut words = Vec::new();
    words.extend(literal_load(1, 8));
    words.push(word(12, 0, 0, 0, 1));
    words.push(header_address);
    for (index, kind) in kinds.iter().enumerate() {
        if !matches!(kind, SectionKind::Data | SectionKind::Bss) {
            continue;
        }
        let record = (32 + 24 * index) as i16;
        words.extend([
            word(7, 2, 1, 0, record + 12), // RAM destination
            word(7, 3, 1, 0, record + 4),
            word(7, 4, 1, 0, record + 8),  // remaining file bytes
            word(7, 5, 1, 0, record + 16), // remaining memory bytes
            word(0, 3, 1, 3, 0),           // absolute ROM source
            word(12, 0, 4, 0, 7),
            word(4, 6, 3, 0, 0),
            word(8, 6, 2, 0, 0),
            word(0, 2, 2, 0, 1),
            word(0, 3, 3, 0, 1),
            word(0, 4, 4, 0, -1),
            word(0, 5, 5, 0, -1),
            word(12, 0, 0, 0, -8),
            word(12, 0, 5, 0, 4),
            word(8, 0, 2, 0, 0),
            word(0, 2, 2, 0, 1),
            word(0, 5, 5, 0, -1),
            word(12, 0, 0, 0, -5),
        ]);
    }
    words.extend([
        word(7, 14, 1, 0, 24),
        word(7, 12, 1, 0, 20),
        word(12, 0, 0, 12, 0),
    ]);
    words.into_iter().flat_map(u32::to_le_bytes).collect()
}

fn base(plan: &LinkPlan, prefix: u32) -> Result<u32, LinkError> {
    plan.read_only
        .map_or(plan.memory_base, |rom| rom.base)
        .checked_add(prefix)
        .ok_or_else(|| link_error("executable base overflow"))
}

pub(super) fn place(
    sections: &mut [LinkedSection],
    plan: &LinkPlan,
    prefix: u32,
) -> Result<u32, LinkError> {
    let base = base(plan, prefix)?;
    let mut file = 32 + 24 * sections.iter().filter(|s| s.memory_size != 0).count() as u32;
    let mut ram = plan.memory_base;
    for section in sections {
        if section.memory_size == 0 {
            let cursor = if plan.read_only.is_some()
                && matches!(section.kind, SectionKind::Text | SectionKind::Rodata)
            {
                base.checked_add(file)
                    .ok_or_else(|| link_error("file address overflow"))?
            } else {
                ram
            };
            section.load_address = align_up(cursor, section.alignment)?;
            continue;
        }
        let stored = !section.data.is_empty();
        if stored {
            let address = align_up(
                base.checked_add(file)
                    .ok_or_else(|| link_error("file address overflow"))?,
                section.alignment,
            )?;
            file = address
                .checked_sub(base)
                .ok_or_else(|| link_error("file offset overflow"))?;
            if plan.read_only.is_none()
                || matches!(section.kind, SectionKind::Text | SectionKind::Rodata)
            {
                section.load_address = address;
            } else {
                ram = align_up(ram, section.alignment)?;
                section.load_address = ram;
                ram = ram
                    .checked_add(section.memory_size)
                    .ok_or_else(|| link_error("RAM section overflow"))?;
            }
            file = file
                .checked_add(section.data.len() as u32)
                .ok_or_else(|| link_error("file size overflow"))?;
            if plan.read_only.is_none() {
                ram = base
                    .checked_add(file)
                    .ok_or_else(|| link_error("RAM copy overflow"))?;
            }
        } else {
            ram = align_up(ram, section.alignment)?;
            section.load_address = ram;
            ram = ram
                .checked_add(section.memory_size)
                .ok_or_else(|| link_error("RAM section overflow"))?;
        }
    }
    if let Some(rom) = plan.read_only
        && prefix.checked_add(file).is_none_or(|size| size > rom.size)
    {
        return Err(link_error("executable does not fit in ROM"));
    }
    align_up(ram, 4)
}

pub(super) fn wire(
    image: &LinkedImage,
    plan: &LinkPlan,
    prefix: u32,
) -> Result<Executable, LinkError> {
    let base = base(plan, prefix)?;
    let mut file = 32 + 24 * image.sections.iter().filter(|s| s.memory_size != 0).count() as u32;
    let mut sections = Vec::new();
    for section in &image.sections {
        if section.memory_size == 0 {
            continue;
        }
        let file_offset = if section.data.is_empty() {
            0
        } else {
            let address = align_up(
                base.checked_add(file)
                    .ok_or_else(|| link_error("file address overflow"))?,
                section.alignment,
            )?;
            let offset = address - base;
            file = offset
                .checked_add(section.data.len() as u32)
                .ok_or_else(|| link_error("file size overflow"))?;
            offset
        };
        sections.push(ExecutableSection {
            kind: section.kind,
            file_offset,
            memory_address: section.load_address,
            memory_size: section.memory_size,
            alignment: section.alignment,
            data: section.data.clone(),
        });
    }
    let executable = Executable {
        entry_address: image.entry_point,
        initial_sp: image.stack_top,
        sections,
    };
    executable.write().map_err(link_error)?;
    Ok(executable)
}
