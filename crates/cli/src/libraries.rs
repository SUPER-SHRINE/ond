//! Compatibility adapter for the compiler-core library loader.
use crate::DriverResult;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn load(project: &Path, roots: &[PathBuf]) -> DriverResult<BTreeMap<String, String>> {
    ond_compiler_core::load_library_sources(project, roots, &BTreeMap::new())
        .map(|sources| {
            sources
                .into_iter()
                .map(|(name, source)| (name, source.text))
                .collect()
        })
        .map_err(|error| {
            error
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect()
        })
}
