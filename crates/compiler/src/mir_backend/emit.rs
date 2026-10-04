//! Encoding and fixed-footprint near/far branch fixups.
use super::*;
use crate::kagura_encoding::{self as encoding, word};
#[derive(Default)]
pub(super) struct Emitter {
    pub words: Vec<u32>,
    labels: Vec<Option<usize>>,
    branches: Vec<(usize, usize, Span)>,
    relocations: Vec<Relocation>,
}
impl Emitter {
    pub fn offset(&self, span: Span) -> Result<u32, Diagnostic> {
        self.words
            .len()
            .checked_mul(4)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| error(span, "text exceeds 32-bit size"))
    }
    pub fn emit(&mut self, op: u32, d: u8, a: u8, b: u8, imm: i16) {
        self.words.push(word(op, d, a, b, imm));
    }
    pub fn label(&mut self) -> usize {
        let n = self.labels.len();
        self.labels.push(None);
        n
    }
    pub fn bind(&mut self, label: usize) {
        self.labels[label] = Some(self.words.len());
    }
    pub fn branch(&mut self, condition: u8, target: usize, span: Span) {
        self.branches.push((self.words.len(), target, span));
        // r12/r13 must be dead on the taken edge. The island is always skipped
        // on fallthrough, and its reserved size is included in every label.
        self.words.extend(encoding::local_slot(condition));
    }
    pub fn immediate(&mut self, reg: u8, bits: u32) {
        // Base 256 digits avoid sign-extending the low half of a 32-bit constant.
        self.emit(0, reg, 0, 0, (bits >> 24) as i16);
        for shift in [16, 8, 0] {
            self.emit(2, reg, reg, 0, 8);
            self.emit(0, reg, reg, 0, ((bits >> shift) & 255) as i16);
        }
    }
    pub fn symbol(&mut self, reg: u8, name: &str, span: Span) -> Result<(), Diagnostic> {
        // Inline address literal: capture PC, load literal, skip its data word.
        self.words.extend(encoding::literal_load(reg, 8));
        self.emit(12, 0, 0, 0, 1);
        self.relocations.push(Relocation {
            section: SectionKind::Text,
            offset: self.offset(span)?,
            kind: RelocationKind::Abs32,
            target: name.into(),
            addend: 0,
        });
        self.words.push(0);
        Ok(())
    }
    pub fn call(&mut self, name: &str, span: Span) -> Result<(), Diagnostic> {
        self.relocations.push(Relocation {
            section: SectionKind::Text,
            offset: self.offset(span)?,
            kind: RelocationKind::CallSlot,
            target: name.into(),
            addend: 0,
        });
        self.words.extend(encoding::link_slot(true, 0));
        Ok(())
    }
    pub fn load(&mut self, reg: u8, offset: u32) {
        self.stack_access(7, reg, offset);
    }
    pub fn store(&mut self, reg: u8, offset: u32) {
        self.stack_access(11, reg, offset);
    }
    /// Large accesses reserve r13 (r12 when the data register is r13).
    /// In particular, storing an incoming stack argument in r12 must preserve it.
    fn stack_access(&mut self, op: u32, reg: u8, offset: u32) {
        if let Ok(imm) = i16::try_from(offset) {
            self.emit(op, reg, 14, 0, imm);
        } else {
            let scratch = if reg == 13 { 12 } else { 13 };
            self.stack_address(scratch, offset);
            self.emit(op, reg, scratch, 0, 0);
        }
    }
    /// Form SP+offset without clobbering any register except the destination.
    pub fn stack_address(&mut self, reg: u8, offset: u32) {
        assert!(reg != 0 && reg != 14);
        if let Ok(imm) = i16::try_from(offset) {
            self.emit(0, reg, 14, 0, imm);
        } else {
            self.immediate(reg, offset);
            self.emit(0, reg, 14, reg, 0);
        }
    }
    pub fn release_frame(&mut self, size: u32) {
        if let Ok(imm) = i16::try_from(size) {
            self.emit(0, 14, 14, 0, imm);
        } else {
            self.immediate(13, size);
            self.emit(0, 14, 14, 13, 0);
        }
    }
    pub fn trap(&mut self) {
        self.emit(6, 0, 0, 0, 0);
    }
    pub fn trap_if_nonzero(&mut self, reg: u8) {
        self.emit(12, 0, reg, 0, 1);
        self.trap();
    }
    pub fn finish(mut self) -> Result<(Vec<u8>, Vec<Relocation>), Diagnostic> {
        self.offset(Span::synthetic())?;
        for (at, target, span) in self.branches {
            let target = self.labels[target].ok_or_else(|| error(span, "unbound block label"))?;
            let delta = target as i64 - at as i64 - 1;
            let imm = if let Ok(imm) = i16::try_from(delta) {
                imm
            } else {
                // Modular byte delta plus captured PC gives the final address
                // at any placement; the enclosing text size was checked above.
                let bytes = (target as i64 - (at as i64 + 3)) * 4;
                self.words[at + 2..at + encoding::LOCAL_SLOT_WORDS]
                    .copy_from_slice(&encoding::relative_thunk(bytes as u32));
                1
            };
            self.words[at] = (self.words[at] & 0xffff0000) | imm as u16 as u32;
        }
        Ok((
            self.words.into_iter().flat_map(u32::to_le_bytes).collect(),
            self.relocations,
        ))
    }
}
