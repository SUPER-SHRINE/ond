//! Process/artifact contract only: no frontend, MIR, or cartridge dependencies.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
mod bytes;
pub mod debug;
mod enum_wire;
pub mod executable;
pub mod link;
pub mod object;
pub use ond_interface as interface;

pub const REVISION: u32 = 7;
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
#[serde(deny_unknown_fields)]
pub struct ProgramMetadata {
    pub entry_symbol: String,
    pub initializers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub package: String,
    pub build_id: String,
    pub path: PathBuf,
    pub content_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageArtifact {
    pub interface: ond_interface::Interface,
    pub contract: Contract,
    pub compiler_id: String,
    pub package: String,
    pub build_id: String,
    #[serde(deserialize_with = "crate::unique_map")]
    pub dependencies: BTreeMap<String, String>,
    pub imports: Vec<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub initializer: Option<String>,
    pub object: object::Object,
    pub debug: debug::DebugObject,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilationManifest {
    pub contract: Contract,
    pub compiler_id: String,
    /// Deterministic Object order, separate from initialization order.
    pub packages: Vec<ArtifactRef>,
    pub program: ProgramMetadata,
}

pub(crate) fn required_nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d)
}

pub(crate) fn unique_map<'de, D, T>(d: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Unique<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Unique<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a map with unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry()? {
                if result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate map key"));
                }
            }
            Ok(result)
        }
    }
    d.deserialize_map(Unique(std::marker::PhantomData))
}
