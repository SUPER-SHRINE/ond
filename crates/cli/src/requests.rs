//! In-process orchestration only; these are not serialized CLI requests.
use ond_protocol::{CompilationManifest, Contract, link};
use std::{collections::BTreeMap, path::PathBuf};
#[derive(Debug, Clone)]
pub struct CompileRequest {
    pub excluded_dirs: Vec<PathBuf>,
    pub contract: Contract,
    pub project_root: PathBuf,
    /// Project-relative .ond paths. Existing files must have identical contents.
    pub sources: BTreeMap<String, String>,
    /// Optional SDK/library sources: include their packages only when imported.
    pub libraries: BTreeMap<String, String>,
    /// Defaults to project_root/target/ond-cache.
    pub cache_dir: Option<PathBuf>,
}
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub manifest: CompilationManifest,
    pub rebuilt: Vec<String>,
    pub reused: Vec<String>,
    pub frontend_ran: bool,
}
#[derive(Debug, Clone)]
pub struct LinkRequest {
    pub manifest: CompilationManifest,
    pub layout: link::LinkPlan,
    pub main_return_address: u32,
}

#[derive(Debug, Clone)]
pub struct LinkResult {
    pub image: link::LinkedImage,
    pub debug: ond_protocol::debug::DebugInfo,
}
