//! Host-side incremental build orchestration. MIR never crosses this boundary.
mod cache;
mod compile;
mod dependencies;
mod graph;
mod input;
mod requests;
pub use requests::{CompileRequest, CompileResult, LinkRequest, LinkResult};
mod libraries;
mod link;
pub use cache::atomic_write as write_artifact;
pub use compile::compile;
pub use dependencies::{ResolvedDependencies, resolve_dependencies, resolve_project_sources};
pub use libraries::load as library_sources;
pub use link::{link, link_with_debug};
pub type DriverResult<T> = Result<T, Vec<String>>;

pub fn compiler_identity() -> DriverResult<String> {
    let executable = std::env::current_exe().map_err(|e| vec![e.to_string()])?;
    let bytes = std::fs::read(executable).map_err(|e| vec![e.to_string()])?;
    Ok(cache::hash(&bytes))
}
#[cfg(test)]
mod export_tests;
#[cfg(test)]
mod tests;
