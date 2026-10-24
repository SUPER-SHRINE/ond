use crate::DriverResult;
use ond_protocol::link::{LinkPlan, MemoryRegion};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub compile: CompileSettings,
    pub link: Option<LinkSettings>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CompileSettings {
    #[serde(default = "compile_output")]
    pub output: PathBuf,
    #[serde(default = "compile_test_output")]
    pub test_output: PathBuf,
    #[serde(default = "cache_dir")]
    pub cache_dir: PathBuf,
}
fn compile_output() -> PathBuf {
    "target/build.ondbuild".into()
}
fn compile_test_output() -> PathBuf {
    "target/test.ondbuild".into()
}
fn cache_dir() -> PathBuf {
    "target/ond-cache".into()
}
fn link_output() -> PathBuf {
    "target/a.bin".into()
}
fn link_test_output() -> PathBuf {
    "target/test.bin".into()
}
impl Default for CompileSettings {
    fn default() -> Self {
        Self {
            output: compile_output(),
            test_output: compile_test_output(),
            cache_dir: cache_dir(),
        }
    }
}
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Startup {
    External,
    #[serde(rename = "self")]
    SelfStarting,
}
#[derive(Debug, Clone, Copy)]
pub enum Exit {
    Loop,
    Jump { address: u32 },
}
impl<'de> Deserialize<'de> for Exit {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Table {
            mode: String,
            address: Option<u32>,
        }
        let table = Table::deserialize(deserializer)?;
        match (table.mode.as_str(), table.address) {
            ("loop", None) => Ok(Self::Loop),
            ("jump", Some(address)) => Ok(Self::Jump { address }),
            _ => Err(serde::de::Error::custom(
                "exit requires mode=loop without address, or mode=jump with address",
            )),
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct LinkSettings {
    #[serde(default = "link_output")]
    pub output: PathBuf,
    #[serde(default = "link_test_output")]
    pub test_output: PathBuf,
    pub startup: Startup,
    pub stack_size: u32,
    pub rom: Option<MemoryRegion>,
    pub ram: MemoryRegion,
    pub exit: Exit,
}
impl LinkSettings {
    pub fn layout(&self) -> DriverResult<LinkPlan> {
        let ram_end = self
            .ram
            .base
            .checked_add(self.ram.size)
            .ok_or_else(|| vec!["RAM address overflow".into()])?;
        if self.ram.size == 0
            || self.stack_size == 0
            || self.stack_size >= self.ram.size
            || !self.ram.base.is_multiple_of(4)
            || !ram_end.is_multiple_of(8)
            || !self.stack_size.is_multiple_of(4)
        {
            return Err(vec!["invalid RAM/stack configuration".into()]);
        }
        if let Some(rom) = self.rom {
            let end = rom
                .base
                .checked_add(rom.size)
                .ok_or_else(|| vec!["ROM address overflow".into()])?;
            if rom.size == 0
                || !rom.base.is_multiple_of(4)
                || !end.is_multiple_of(4)
                || rom.base < ram_end && end > self.ram.base
            {
                return Err(vec!["invalid or overlapping ROM region".into()]);
            }
        }
        if self.startup == Startup::SelfStarting && self.rom.is_none_or(|rom| rom.base != 0) {
            return Err(vec!["self startup requires ROM with base 0".into()]);
        }
        if matches!(self.exit, Exit::Jump { address } if !address.is_multiple_of(4)) {
            return Err(vec!["exit address must be word-aligned".into()]);
        }
        Ok(LinkPlan {
            read_only: self.rom,
            memory_base: self.ram.base,
            memory_size: self.ram.size,
            stack_size: self.stack_size,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        })
    }
}
pub fn read_settings(root: &Path) -> DriverResult<Settings> {
    let path = root.join("ond.toml");
    let text =
        std::fs::read_to_string(&path).map_err(|e| vec![format!("{}: {e}", path.display())])?;
    let settings: Settings =
        toml::from_str(&text).map_err(|e| vec![format!("{}: {e}", path.display())])?;
    let mut paths = vec![
        &settings.compile.output,
        &settings.compile.test_output,
        &settings.compile.cache_dir,
    ];
    if let Some(link) = &settings.link {
        paths.extend([&link.output, &link.test_output]);
    }
    if paths.iter().any(|p| p.as_os_str().is_empty()) {
        return Err(vec!["output/cache paths must be nonempty".into()]);
    }
    Ok(settings)
}
