use crate::object::SectionKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// RAM layout with an optional disjoint region for executable/read-only bytes.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LinkPlan {
    /// Optional separate read-only region for Text/Rodata. Data/Bss remain in RAM.
    #[serde(deserialize_with = "crate::required_nullable")]
    pub read_only: Option<MemoryRegion>,
    pub memory_base: u32,
    pub memory_size: u32,
    pub stack_size: u32,
    pub entry_symbol: String,
    pub heap_base_symbol: String,
    pub stack_bottom_symbol: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct MemoryRegion {
    pub base: u32,
    pub size: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LinkedSection {
    pub kind: SectionKind,
    pub alignment: u32,
    pub load_address: u32,
    pub memory_size: u32,
    #[serde(with = "crate::bytes")]
    pub data: Vec<u8>,
}

/// Logical link result; executable files use `crate::executable`, not JSON.
#[derive(Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LinkedImage {
    pub build_id: String,
    pub entry_point: u32,
    pub ram_size: u32,
    pub stack_size: u32,
    pub sections: Vec<LinkedSection>,
    pub heap_base: u32,
    pub stack_bottom: u32,
    pub stack_top: u32,
    #[serde(deserialize_with = "crate::unique_map")]
    pub symbols: BTreeMap<String, u32>,
}
