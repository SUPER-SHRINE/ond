//! Host-side incremental build orchestration. MIR never crosses this boundary.
mod cache;
mod json;

mod compile;
mod dependencies;
mod graph;
mod input;
mod requests;
pub mod settings;
pub use requests::{CompileRequest, CompileResult, LinkRequest, LinkResult};
mod link;
pub use cache::atomic_write as write_artifact;
pub use compile::compile;
pub use dependencies::{ResolvedDependencies, resolve_dependencies, resolve_project_sources};
pub use link::{link, link_executable, link_with_debug};
pub type DriverResult<T> = Result<T, Vec<String>>;

pub fn compiler_identity() -> DriverResult<String> {
    let executable = std::env::current_exe().map_err(|e| vec![e.to_string()])?;
    let bytes = std::fs::read(executable).map_err(|e| vec![e.to_string()])?;
    Ok(cache::hash(&bytes))
}
#[cfg(test)]
mod export_tests;
#[cfg(test)]
use ond_compiler_core::mir;
#[cfg(test)]
mod generation_tests;
#[cfg(test)]
#[path = "../../compiler-core/src/memory_test_vm.rs"]
mod memory_vm;
#[cfg(test)]
mod role_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod dependency_test_support;
