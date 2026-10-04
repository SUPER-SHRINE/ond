//! Process/artifact contract only: no frontend, MIR, or cartridge dependencies.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
mod bytes;
pub mod debug;
pub mod link;
pub mod object;
pub use ond_interface as interface;

pub const REVISION: u32 = 4;
pub const TARGET: &str = "kagura-v1";
pub const ABI: &str = "ond-kagura-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub revision: u32,
    pub target: String,
    pub abi: String,
}
impl Default for Contract {
    fn default() -> Self {
        Self {
            revision: REVISION,
            target: TARGET.into(),
            abi: ABI.into(),
        }
    }
}
impl Contract {
    pub fn validate(&self) -> Result<(), String> {
        if self != &Self::default() {
            return Err(format!("unsupported Ond contract: {self:?}"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramMetadata {
    pub entry_symbol: String,
    pub initializers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub package: String,
    pub build_id: String,
    pub path: PathBuf,
    pub content_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageArtifact {
    pub interface: ond_interface::Interface,
    pub contract: Contract,
    pub compiler_id: String,
    pub package: String,
    pub build_id: String,
    pub dependencies: BTreeMap<String, String>,
    pub imports: Vec<String>,
    pub initializer: Option<String>,
    pub object: object::Object,
    pub debug: debug::DebugObject,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationManifest {
    pub contract: Contract,
    pub compiler_id: String,
    /// Deterministic Object order, separate from initialization order.
    pub packages: Vec<ArtifactRef>,
    pub program: ProgramMetadata,
}
