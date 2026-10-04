//! Patch preallocated islands, never insert bytes into already laid-out text.
use super::*;
use crate::{kagura_encoding as encoding, object::Relocation};

pub(super) fn patch(
    object: &Object,
    relocation: &Relocation,
    section: &mut LinkedSection,
    offset: u32,
    target: u32,
) -> Result<(), LinkError> {
    if relocation.section != SectionKind::Text || relocation.offset % 4 != 0 {
        return Err(link_error("branch slot must be word-aligned in text"));
    }
    let input = object
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::Text)
        .ok_or_else(|| link_error("branch slot has no input text"))?;
    let end = relocation
        .offset
        .checked_add((encoding::LINK_SLOT_WORDS * 4) as u32)
        .ok_or_else(|| link_error("branch slot offset overflow"))?;
    if end > input.memory_size || end as usize > input.data.len() {
        return Err(link_error("branch slot exceeds input text bounds"));
    }
    let first = read_u32(section, offset)?;
    let call = relocation.kind == RelocationKind::CallSlot;
    let condition = ((first >> 20) & 15) as u8;
    if call && condition != 0 {
        return Err(link_error("call slot must be unconditional"));
    }
    let template = encoding::link_slot(call, condition);
    for (i, expected) in template.into_iter().enumerate() {
        if read_u32(section, offset + i as u32 * 4)? != expected {
            return Err(link_error("invalid or overlapping branch slot template"));
        }
    }
    let target = u32::try_from(i64::from(target) + i64::from(relocation.addend))
        .map_err(|_| link_error("branch slot target address overflow"))?;
    if target % 4 != 0 {
        return Err(link_error("branch relocation target is not word-aligned"));
    }
    let site = section
        .load_address
        .checked_add(offset)
        .ok_or_else(|| link_error("branch patch address overflow"))?;
    let delta = (i64::from(target) - (i64::from(site) + 4)) / 4;
    if site % 4 != 0 {
        return Err(link_error("branch slot site is not word-aligned"));
    }
    let imm = if let Ok(imm) = i16::try_from(delta) {
        imm
    } else {
        for (i, instruction) in encoding::absolute_thunk(target).into_iter().enumerate() {
            write_u32(section, offset + (i as u32 + 2) * 4, instruction)?;
        }
        1
    };
    write_u32(section, offset, (first & 0xffff0000) | imm as u16 as u32)
}
