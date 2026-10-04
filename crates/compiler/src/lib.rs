#![forbid(unsafe_code)]

pub mod kagura_encoding;
pub mod linker;
pub mod mir_backend;
mod object;
pub mod pipeline;
pub mod program;
#[cfg(test)]
mod test_support;

pub use ond_compiler_core::{diagnostic, ond, project, source};

pub mod ir {
    pub use ond_compiler_core::ir::*;

    pub mod object {
        pub use crate::object::*;
    }
}

pub use diagnostic::{Diagnostic, FrontendError, Severity};
pub use ond::parse_source_file;
pub use pipeline::{
    Compilation, compile_loaded_project, compile_project, compile_project_with_overrides,
};
pub use project::{LoadedProject, PackageId, load_project, load_project_with_overrides};
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectConfig {
    pub ram_size: u32,
    pub stack_size: u32,
}

#[cfg(test)]
impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            ram_size: 16 * 1024 * 1024,
            stack_size: 1024 * 1024,
        }
    }
}
