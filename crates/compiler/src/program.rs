//! Ond startup semantics kept outside target Object data.
use crate::{
    diagnostic::{Diagnostic, FrontendError},
    source::Span,
};
use ond_compiler_core::mir;
mod startup;
pub use startup::program_startup;
#[derive(Debug, Clone)]
pub struct ProgramMetadata {
    pub entry_symbol: String,
    pub initializers: Vec<String>,
}
impl ProgramMetadata {
    pub fn from_mir(project: &mir::Project) -> Result<Self, FrontendError> {
        mir::validate_project(project).map_err(|e| {
            FrontendError::new(vec![Diagnostic::error(
                Span::synthetic(),
                format!("invalid MIR: {e:?}"),
            )])
        })?;
        Ok(Self {
            entry_symbol: "main.main".into(),
            initializers: project
                .initialization_order
                .iter()
                .filter_map(|id| {
                    project
                        .packages
                        .iter()
                        .find(|p| p.id == *id)
                        .and_then(|p| p.initializer.as_ref())
                        .map(|f| f.name.clone())
                })
                .collect(),
        })
    }
}
