//! Ond startup semantics kept outside target Object data.
use crate::{
    diagnostic::{Diagnostic, FrontendError},
    source::Span,
};
use ond_compiler_core::mir;
mod startup;
pub use startup::program_startup;

pub(crate) fn initializer_symbol(package: &mir::Package) -> Option<String> {
    package
        .initializer
        .as_ref()
        .map(|function| function.name.clone())
        .or_else(|| {
            package
                .globals
                .iter()
                .any(|global| global.initializer.is_some())
                .then(|| format!("{}.\u{1f}global.init", package.logical_path))
        })
}
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
        fn visit(
            id: ond_compiler_core::PackageId,
            project: &mir::Project,
            seen: &mut std::collections::BTreeSet<ond_compiler_core::PackageId>,
            order: &mut Vec<String>,
        ) {
            if !seen.insert(id) {
                return;
            }
            let Some(package) = project.packages.iter().find(|package| package.id == id) else {
                return;
            };
            for dependency in &package.imports {
                visit(*dependency, project, seen, order);
            }
            if let Some(symbol) = initializer_symbol(package) {
                order.push(symbol);
            }
        }
        let mut initializers = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        if let Some(root) = project.packages.iter().find(|package| {
            package
                .functions
                .iter()
                .any(|function| function.name == "main.main")
        }) {
            visit(root.id, project, &mut seen, &mut initializers);
        }
        Ok(Self {
            entry_symbol: "main.main".into(),
            initializers,
        })
    }
}
