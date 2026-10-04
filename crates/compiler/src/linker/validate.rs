//! Structural validation before layout allocates or patches any bytes.
use super::*;
use crate::ir::object::{SymbolBinding, SymbolKind};
use std::collections::BTreeSet;

pub(super) fn validate(objects: &[Object]) -> Result<(), LinkError> {
    for o in objects {
        let fail = |message: &str| link_error(format!("object {}: {message}", o.name));
        let mut kinds = BTreeSet::new();
        for s in &o.sections {
            if !kinds.insert(s.kind) {
                return Err(fail("duplicate section kind"));
            }
            if !s.alignment.is_power_of_two() {
                return Err(fail("invalid section alignment"));
            }
            if s.data.len() as u64 > u64::from(s.memory_size) {
                return Err(fail("payload exceeds memory size"));
            }
            if s.kind == SectionKind::Bss && !s.data.is_empty() {
                return Err(fail("Bss has payload"));
            }
            if matches!(s.kind, SectionKind::Text | SectionKind::Rodata)
                && s.data.len() as u64 != u64::from(s.memory_size)
            {
                return Err(fail("read-only section requires complete payload"));
            }
            if s.kind == SectionKind::Text && (s.alignment < 4 || s.memory_size % 4 != 0) {
                return Err(fail("text must be word-aligned"));
            }
        }
        let mut names = BTreeSet::new();
        for s in &o.symbols {
            if s.name.is_empty() || !names.insert(&s.name) {
                return Err(fail("empty or duplicate symbol"));
            }
            if s.binding == SymbolBinding::Imported {
                if s.section.is_some() || s.offset != 0 || s.size != 0 {
                    return Err(fail("invalid imported symbol"));
                }
                continue;
            }
            let section = s
                .section
                .and_then(|k| o.sections.iter().find(|x| x.kind == k))
                .ok_or_else(|| fail("symbol refers to missing section"))?;
            if u64::from(s.offset) + u64::from(s.size) > u64::from(section.memory_size) {
                return Err(fail("symbol exceeds section bounds"));
            }
            if s.kind == SymbolKind::Function
                && (s.section != Some(SectionKind::Text)
                    || s.offset % 4 != 0
                    || s.offset >= section.memory_size)
            {
                return Err(fail("invalid function symbol"));
            }
        }
        let mut ranges = Vec::new();
        for r in &o.relocations {
            let s = o
                .sections
                .iter()
                .find(|s| s.kind == r.section)
                .ok_or_else(|| fail("relocation refers to missing section"))?;
            let slot = matches!(r.kind, RelocationKind::CallSlot | RelocationKind::JumpSlot);
            let width = if slot { 24 } else { 4 };
            let end = r
                .offset
                .checked_add(width)
                .ok_or_else(|| fail("relocation range overflow"))?;
            if r.offset % 4 != 0 {
                return Err(fail("relocation must be word-aligned"));
            }
            if end as u64 > s.data.len() as u64 {
                return Err(fail(
                    "relocation exceeds input text bounds or payload bounds",
                ));
            }
            if r.kind != RelocationKind::Abs32 && r.section != SectionKind::Text {
                return Err(fail("branch relocation must be in text"));
            }
            if ranges
                .iter()
                .any(|&(k, a, b)| k == r.section && r.offset < b && end > a)
            {
                return Err(fail("overlapping relocation/template ranges"));
            }
            if slot
                && o.symbols
                    .iter()
                    .any(|x| x.section == Some(r.section) && x.offset > r.offset && x.offset < end)
            {
                return Err(fail("symbol enters branch slot interior"));
            }
            ranges.push((r.section, r.offset, end));
        }
    }
    Ok(())
}
