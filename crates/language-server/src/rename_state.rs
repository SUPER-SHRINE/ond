use std::path::Path;

use compiler::ir::ast;
use compiler::source::SourceDb;
use serde_json::Value;

use crate::protocol::Position;
use crate::rename::rename as rename_symbol;
use crate::rename::{prepare_rename as prepare_symbol_rename, references as symbol_references};
use crate::state::ServerState;
use crate::util::{
    find_project_root, normalize_path, path_from_uri, render_frontend_error, same_path,
};

impl ServerState {
    pub fn prepare_rename(&self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(root) = find_project_root(&path) {
            let (loaded, project) = self.fresh_project(&root)?;
            if let Some((package, file)) = find_document(&project, &loaded, &path) {
                return Ok(prepare_symbol_rename(
                    &project,
                    package,
                    file,
                    &loaded.sources,
                    &Position { line, character },
                    Some(&root),
                )
                .unwrap_or(Value::Null));
            }
            return Ok(Value::Null);
        }
        let Some((sources, file, package, project)) = self.single_file_project(&path)? else {
            return Ok(Value::Null);
        };
        Ok(prepare_symbol_rename(
            &project,
            &package,
            &file,
            &sources,
            &Position { line, character },
            None,
        )
        .unwrap_or(Value::Null))
    }

    pub fn references(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Value, String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(root) = find_project_root(&path) {
            let (loaded, project) = self.fresh_project(&root)?;
            if let Some((package, file)) = find_document(&project, &loaded, &path) {
                return Ok(symbol_references(
                    &project,
                    package,
                    file,
                    &loaded.sources,
                    &Position { line, character },
                    include_declaration,
                ));
            }
            return Ok(Value::Array(Vec::new()));
        }
        let Some((sources, file, package, project)) = self.single_file_project(&path)? else {
            return Ok(Value::Array(Vec::new()));
        };
        Ok(symbol_references(
            &project,
            &package,
            &file,
            &sources,
            &Position { line, character },
            include_declaration,
        ))
    }

    pub fn rename(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Value, String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(root) = find_project_root(&path) {
            let (loaded, project) = self.fresh_project(&root)?;
            if let Some((package, file)) = find_document(&project, &loaded, &path) {
                return rename_symbol(
                    &project,
                    package,
                    file,
                    &loaded.sources,
                    &Position { line, character },
                    Some(&root),
                    new_name,
                );
            }
            return Err("the document is not part of the current Ond project".to_string());
        }
        let Some((sources, file, package, project)) = self.single_file_project(&path)? else {
            return Err("the current document cannot be parsed".to_string());
        };
        rename_symbol(
            &project,
            &package,
            &file,
            &sources,
            &Position { line, character },
            None,
            new_name,
        )
    }

    fn fresh_project(
        &self,
        root: &Path,
    ) -> Result<(compiler::LoadedProject, ast::Project), String> {
        let loaded = self.load_current_project(root)?;
        let project = compiler::ond::parse_project(&loaded).map_err(render_frontend_error)?;
        Ok((loaded, project))
    }

    fn single_file_project(
        &self,
        path: &Path,
    ) -> Result<Option<(SourceDb, ast::File, ast::Package, ast::Project)>, String> {
        let Some(text) = self.document_text(path) else {
            return Ok(None);
        };
        let mut sources = SourceDb::default();
        let file_id = sources.add_file(path.to_path_buf(), text.to_string());
        let Ok(file) = compiler::parse_source_file(file_id, text) else {
            return Ok(None);
        };
        let package = ast::Package {
            logical_path: ".".to_string(),
            files: vec![file.clone()],
        };
        let project = ast::Project {
            packages: vec![package.clone()],
        };
        Ok(Some((sources, file, package, project)))
    }
}

fn find_document<'a>(
    project: &'a ast::Project,
    loaded: &compiler::LoadedProject,
    path: &Path,
) -> Option<(&'a ast::Package, &'a ast::File)> {
    project.packages.iter().find_map(|package| {
        package
            .files
            .iter()
            .find(|file| same_path(loaded.sources.file(file.file_id).path(), path))
            .map(|file| (package, file))
    })
}
