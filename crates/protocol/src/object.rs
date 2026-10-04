use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Object {
    pub name: String,
    pub sections: Vec<Section>,
    pub symbols: Vec<Symbol>,
    pub relocations: Vec<Relocation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Section {
    pub kind: SectionKind,
    pub alignment: u32,
    #[serde(with = "crate::bytes")]
    pub data: Vec<u8>,
    pub memory_size: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SectionKind {
    Text,
    Rodata,
    Data,
    Bss,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub binding: SymbolBinding,
    pub kind: SymbolKind,
    pub section: Option<SectionKind>,
    pub offset: u32,
    pub size: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolBinding {
    Local,
    Exported,
    Imported,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Object,
    Section,
    Runtime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Relocation {
    pub section: SectionKind,
    pub offset: u32,
    pub kind: RelocationKind,
    pub target: String,
    pub addend: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationKind {
    Abs32,
    Call16,
    Jump16,
    /// Six-word call site with a private, linker-owned r12 thunk island.
    CallSlot,
    /// Six-word conditional/unconditional jump site; taken path may clobber r12.
    JumpSlot,
}
