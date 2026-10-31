//! Cursor-specific declaration environments shared by editor operations.
use crate::hover_reference::ResolvedReference;
use compiler::{
    ir::ast,
    source::{SourceDb, Span},
};

pub(crate) fn project_at(project: &ast::Project, sources: &SourceDb, caller: Span) -> ast::Project {
    let mut project = project.clone();
    for package in &mut project.packages {
        for file in &mut package.files {
            let keep = |span: Span| {
                let owner = sources.roles(span.file);
                let reference = sources.roles(caller.file);
                if owner
                    .zip(reference)
                    .is_some_and(|(owner, reference)| owner.project != reference.project)
                {
                    return owner.is_none_or(|owner| owner.selected(span.start));
                }
                compiler::roles::visible(sources, span, caller)
            };
            file.imports.retain(|import| keep(import.span));
            file.decls
                .retain(|declaration| keep(compiler::ond::declaration_span(declaration)));
        }
    }
    project
}

pub(crate) fn resolve_reference(
    project: &ast::Project,
    sources: &SourceDb,
    file: &ast::File,
    offset: usize,
) -> Option<ResolvedReference> {
    if sources.files().iter().enumerate().all(|(index, _)| {
        sources
            .roles(compiler::source::FileId(index))
            .is_none_or(|roles| roles.configuration.is_none())
    }) {
        return crate::hover_reference::resolve_reference(
            project,
            project.packages.iter().find(|package| {
                package
                    .files
                    .iter()
                    .any(|candidate| candidate.file_id == file.file_id)
            })?,
            file,
            offset,
        );
    }
    let project = project_at(project, sources, Span::new(file.file_id, offset, offset));
    let (package, file) = project.packages.iter().find_map(|package| {
        package
            .files
            .iter()
            .find(|candidate| candidate.file_id == file.file_id)
            .map(|file| (package, file))
    })?;
    crate::hover_reference::resolve_reference(&project, package, file, offset)
}
